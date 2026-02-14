#pragma once

#include <grpcpp/support/client_callback.h>

#include "dictype.grpc.pb.h"

class GrpcClient final
    : public grpc::ClientReadReactor<Dictype::TranscribeResponse> {
  public:
    using ResponseCb = std::function<void(const Dictype::TranscribeResponse&)>;
    using DoneCb = std::function<void(const grpc::Status&)>;

    explicit GrpcClient(Dictype::Dictype::Stub* stub, ResponseCb onResponse,
                        DoneCb onDone, const std::string& profileName);

    void OnDone(const grpc::Status& s) override;
    void OnReadDone(bool ok) override;

  private:
    grpc::ClientContext context_;

    Dictype::TranscribeResponse response_;

    ResponseCb onResponse_;
    DoneCb onDone_;
};
